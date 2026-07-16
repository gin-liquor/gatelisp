library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_reverse_bits is
  generic (
    gl_g0 : positive := 10
  );
  port (
    gl_p0_input : in unsigned(gl_g0 - 1 downto 0);
    gl_p1_value : out unsigned(gl_g0 - 1 downto 0)
  );
end entity gl_m0_generic_reverse_bits;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_reverse_bits is
  signal gl_s1_value : unsigned(gl_g0 - 1 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_reverse_bits(value : unsigned) return unsigned is
    variable result : unsigned(value'range);
  begin
    for offset in 0 to value'length - 1 loop
      result(result'low + offset) := value(value'high - offset);
    end loop;
    return result;
  end function gl_reverse_bits;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_value <= gl_reverse_bits(gl_p0_input);
  end process gl_comb_0;
  gl_p1_value <= gl_s1_value;
end architecture rtl;
