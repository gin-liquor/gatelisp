library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_concat_bits is
  port (
    gl_p0_valid : in std_logic;
    gl_p1_payload : in unsigned(6 downto 0);
    gl_p2_value : out unsigned(7 downto 0)
  );
end entity gl_m0_concat_bits;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_concat_bits is
  signal gl_s2_value : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_bit_to_slv(value : std_logic) return std_logic_vector is
  begin
    return std_logic_vector'(0 => value);
  end function gl_bit_to_slv;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_value <= unsigned((gl_bit_to_slv(gl_p0_valid) & std_logic_vector(gl_p1_payload)));
  end process gl_comb_0;
  gl_p2_value <= gl_s2_value;
end architecture rtl;
