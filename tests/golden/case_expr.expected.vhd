library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_case_expression is
  port (
    gl_p0_selector : in unsigned(1 downto 0);
    gl_p1_a : in unsigned(7 downto 0);
    gl_p2_b : in unsigned(7 downto 0);
    gl_p3_c : in unsigned(7 downto 0);
    gl_p4_value : out unsigned(7 downto 0);
    gl_p5_low : out std_logic
  );
end entity gl_m0_case_expression;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_case_expression is
  signal gl_s4_value : unsigned(7 downto 0);
  signal gl_s5_low : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_bit_at(value : unsigned; index : natural) return std_logic is
  begin
    return value(index);
  end function gl_bit_at;
begin
  gl_comb_0 : process(all)
    variable gl_tmp_0 : unsigned(1 downto 0);
    variable gl_tmp_1 : unsigned(7 downto 0);
  begin
    gl_tmp_0 := gl_p0_selector;
    if (gl_tmp_0 = resize(unsigned'(x"0000000000000000"), 2)) then
      gl_tmp_1 := gl_p1_a;
    else
      if (gl_tmp_0 = resize(unsigned'(x"0000000000000001"), 2)) then
        gl_tmp_1 := gl_p2_b;
      else
        if (gl_tmp_0 = resize(unsigned'(x"0000000000000002"), 2)) then
          gl_tmp_1 := gl_p3_c;
        else
          gl_tmp_1 := resize(unsigned'(x"0000000000000000"), 8);
        end if;
      end if;
    end if;
    gl_s4_value <= gl_tmp_1;
  end process gl_comb_0;
  gl_comb_1 : process(all)
    variable gl_tmp_0 : unsigned(1 downto 0);
    variable gl_tmp_1 : unsigned(7 downto 0);
  begin
    gl_tmp_0 := gl_p0_selector;
    if (gl_tmp_0 = resize(unsigned'(x"0000000000000000"), 2)) then
      gl_tmp_1 := gl_p1_a;
    else
      if (gl_tmp_0 = resize(unsigned'(x"0000000000000001"), 2)) then
        gl_tmp_1 := gl_p2_b;
      else
        if (gl_tmp_0 = resize(unsigned'(x"0000000000000002"), 2)) then
          gl_tmp_1 := gl_p3_c;
        else
          gl_tmp_1 := resize(unsigned'(x"0000000000000000"), 8);
        end if;
      end if;
    end if;
    gl_s5_low <= gl_bit_at(gl_tmp_1, 0);
  end process gl_comb_1;
  gl_p4_value <= gl_s4_value;
  gl_p5_low <= gl_s5_low;
end architecture rtl;
