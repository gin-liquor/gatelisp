library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_enum_opcode is
  port (
    gl_p0_bits : in unsigned(4 downto 0);
    gl_p1_opcode : out unsigned(4 downto 0);
    gl_p2_is_jump : out std_logic;
    gl_p3_is_nop : out std_logic
  );
end entity gl_m0_enum_opcode;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_enum_opcode is
  constant gl_enum_opcode_nop : unsigned(4 downto 0) := to_unsigned(0, 5);
  constant gl_enum_opcode_jump : unsigned(4 downto 0) := to_unsigned(3, 5);
  constant gl_enum_opcode_jump_if : unsigned(4 downto 0) := to_unsigned(4, 5);
  signal gl_s1_opcode : unsigned(4 downto 0);
  signal gl_s2_is_jump : std_logic;
  signal gl_s3_is_nop : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_opcode <= gl_p0_bits;
  end process gl_comb_0;
  gl_comb_1 : process(all)
    variable gl_tmp_0 : unsigned(4 downto 0);
    variable gl_tmp_1 : std_logic;
  begin
    gl_tmp_0 := gl_s1_opcode;
    if (gl_tmp_0 = gl_enum_opcode_jump) then
      gl_tmp_1 := '1';
    else
      if (gl_tmp_0 = gl_enum_opcode_jump_if) then
        gl_tmp_1 := '1';
      else
        gl_tmp_1 := '0';
      end if;
    end if;
    gl_s2_is_jump <= gl_tmp_1;
  end process gl_comb_1;
  gl_comb_2 : process(all)
    variable gl_tmp_0 : unsigned(4 downto 0);
    variable gl_tmp_1 : std_logic;
  begin
    gl_tmp_0 := gl_s1_opcode;
    if (gl_tmp_0 = gl_enum_opcode_nop) then
      gl_tmp_1 := '1';
    else
      gl_tmp_1 := '0';
    end if;
    gl_s3_is_nop <= gl_tmp_1;
  end process gl_comb_2;
  gl_p1_opcode <= gl_s1_opcode;
  gl_p2_is_jump <= gl_s2_is_jump;
  gl_p3_is_nop <= gl_s3_is_nop;
end architecture rtl;
